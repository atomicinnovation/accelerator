---
type: "work-item"
id: "0228"
title: "Layered Configuration Key Model"
date: "2026-08-30T14:35:09+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "ready"
kind: "story"
priority: "high"
parent: "work-item:0146"
blocks: ["work-item:0229", "work-item:0230"]
relates_to: ["work-item:0220", "work-item:0227"]
external_id: "PP-758"
tags: ["configuration", "work-management", "migration", "tracker"]
last_updated: "2026-09-10T01:44:18+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---
# 0228: Layered Configuration Key Model

**Kind**: Story
**Status**: Ready
**Priority**: High
**Author**: Toby Clemson

## Summary

Rename `work.default_project_code` to `work.key` and the `id_pattern` placeholder
`{project}` to `{key}`, and separate the two concepts the old field conflated. The
tracker-native key (`linear.team_key` / `jira.project_key`) is the canonical,
integration-owned **scope key**; `work.key` is the **local ID prefix**, set explicitly
and fully independent of the scope key. Neither derives from the other — when
`id_pattern` uses `{key}`, `work.key` is required. A tracker-aware read-time alias
plus migration carries existing `default_project_code` / `{project}` configs across
a one-release deprecation window (ships 1.24.0, alias removed 1.25.0), and
`init-linear` / `init-jira` write the discovered scope key into the tracker section.

## Context

`default_project_code` is misnamed: its real job is the key a tracker stamps on
issue identifiers — a Jira project key, a Linear team key — the key of the entity
that owns the issue-number sequence, not a "project". It is also overloaded as the
sync scope, which is what let bug 0220 hide.

Integration skills (search/show for Jira and Linear) must keep working, and may in
future be packaged independently of the accelerator plugin. So the dependency
must run work → integration, never the reverse: integration skills read only their
own `jira:` / `linear:` section; the work layer reads the integration-owned key.

Two independent layerings apply; the title's "layered" names the first. *Ownership*:
the integration section owns the scope key; `work.key` — the local ID prefix — is a
separate value the work layer owns, not derived from it. *Config-file*: the key may be
set in team (`config.md`) or personal
(`config.local.md`) config, resolved by the existing precedence (personal overrides
team). Resolution runs on the effective post-override configuration.

## Requirements

- Rename `work.default_project_code` → `work.key`; `id_pattern` placeholder
  `{project}` → `{key}`.
- Make `linear.team_key` / `jira.project_key` the canonical, integration-owned scope
  key, usable by the integration skills with no `work.*` present.
- `work.key` is the local ID prefix, set explicitly and independent of the scope key.
  It never derives from the scope key; when `id_pattern` references `{key}`, `work.key`
  is required and its absence is a config-validation error (no silent fallback to the
  scope key — that would give local IDs the scope key's value as their prefix instead
  of an independent number sequence, falsely implying they correspond to remote
  issues). Setting `work.key`
  alongside the scope key is not an error, whatever the values — including equal, a
  deliberate choice to mirror the tracker prefix.
- `work.key` (and `{key}`) govern local ID composition only when `id_pattern`
  references `{key}`; a pattern without `{key}` yields tracker-independent local IDs.
  `{key}` aligns the local ID prefix only — never the numeric sequence, and never the
  local-to-remote join, which stays on `external_id`.
- The scope key resolves the tracker's creation-home entity — the single Jira project
  / Linear team new items are minted into — Jira by identity (the key is the project
  key), Linear by catalogue lookup (team key → team UUID). This is the base scope that
  creation and discovery consume; 0229 layers its `pull` block on top of this base
  scope. The resolver already exists — this item routes the renamed field through it,
  it does not build new resolution.
- Provide a temporary read-time alias plus migration for existing
  `default_project_code` and `{project}` configs. The alias is **tracker-aware**: it
  resolves into the scope key (`linear.team_key` / `jira.project_key`) when the repo is
  tracker-backed, and into `work.key` when tracker-less. Where the legacy `id_pattern`
  used `{project}`, the migration also materialises `work.key` from the same legacy
  value so `{key}` keeps resolving and local IDs render identically — but now as an
  explicit, editable value rather than a silent default. The alias is a deprecation
  window: `work.key` ships in 1.24.0, reading a deprecated key emits a warning naming
  the removal release, and the alias is removed in 1.25.0.
- `init-linear` / `init-jira` write the discovered key into the tracker section,
  overwriting any key already present there.

## Acceptance Criteria

- [ ] Given only a `jira:` or `linear:` section with its key and no `work.*`, when
      an integration skill runs, then it resolves its scope key from `linear.team_key`
      / `jira.project_key` and completes its search/show operation without raising a
      missing-`work.*`-config error.
- [ ] Given a repo whose `id_pattern` references `{key}` with `work.key` omitted, when
      configuration is validated, then it fails, naming `work.key` as required by the
      pattern — `{key}` never silently falls back to the scope key.
- [ ] Given a tracker-backed repo whose `id_pattern` references `{key}` with `work.key`
      set, when IDs are minted, then `{key}` resolves from `work.key`, while the
      creation-home entity resolves independently from the scope key (Jira by identity,
      Linear by catalogue lookup).
- [ ] Given `work.key` set to a value different from the scope key, when work items are
      created and a pull runs, then local IDs carry the `work.key` prefix, discovery
      scopes to the entity resolved from the scope key, and no validation error or
      warning is raised.
- [ ] Given `work.key` set equal to the scope key, when configuration is validated,
      then no validation error and no warning are raised — a mirrored prefix is a
      supported, explicit choice.
- [ ] Given a tracker-less repo whose `id_pattern` references `{key}` with `work.key`
      set, when IDs are minted, then `{key}` resolves directly from `work.key`.
- [ ] Given a tracker-backed repo whose `id_pattern` omits `{key}` (e.g.
      `{number:04d}`), when a work item is created, then its local `id` carries no
      tracker prefix and is joined to its remote issue solely on `external_id`.
- [ ] Given the scope key in team config and `work.key` in personal config, when the
      effective configuration is resolved, then personal-over-team precedence applies
      and `{key}` uses the personal `work.key` while discovery scopes from the team
      scope key.
- [ ] Given an existing tracker-backed config using `default_project_code` with an
      `id_pattern` that references `{project}` (e.g. `default_project_code: PP`,
      `id_pattern: {project}-{number:04d}` rendering `PP-0001`), when it is read after
      this change, then the value resolves into the scope key and is materialised as an
      explicit `work.key`, IDs still render `PP-0001`, and a deprecation warning names
      the release (1.25.0) in which the alias is removed.
- [ ] Given an existing tracker-less config using `default_project_code`, when it is
      read, then it resolves into `work.key`, renders IDs identically (a legacy
      `default_project_code: PP` still rendering `PP-0001`), and a deprecation warning
      names the release (1.25.0) in which the alias is removed.
- [ ] Given a tracker section with no key, when `init-linear` / `init-jira` runs, then
      the discovered key is written into the tracker section.
- [ ] Given a tracker section already carries a key, when `init-linear` /
      `init-jira` runs, then the existing key is overwritten with the discovered one.

## Open Questions

- None outstanding. The deprecation-alias removal release is pinned: `work.key` ships
  in 1.24.0 and the alias is removed in 1.25.0.

## Dependencies

- Blocked by: none.
- Sequenced after 0220 (done) — this item renames the `work.default_project_code`
  touchpoint 0220 shipped against, the reconciliation 0220 requested of whichever
  sibling ships second.
- Blocks: 0229 (Per-Tracker Pull Scope Configuration) — 0229's base scope resolves
  from the scope key this item introduces.
- Blocks: 0230 (Tracker-owned work-item ID generation) — 0230 builds on the scope-key
  / `work.key` separation this item establishes.
- Blocks: the `work.key`-validation portion of 0227 (accelerator config validate
  Command) — 0227's `config validate` cannot validate the `work.key`-required rule or
  the deprecated-alias warning until this item defines them, so command-time and
  load-time validation agree. This gates only that portion of 0227, not the whole
  command.

## Assumptions

- The tracker-aware alias selects its target from the active integration
  (`work.integration`), the authoritative "is this repo tracker-backed" signal,
  rather than the mere presence of a `linear:` / `jira:` section.
- Config-file layering reuses the existing team/personal precedence machinery
  (personal overrides team); this item makes the key model participate correctly, it
  introduces no new layering mechanics.
- `work.key` ships in 1.24.0 and the alias is removed in 1.25.0; if the introducing
  release slips, the removal shifts to the following minor to preserve the one-release
  deprecation window.
- Late tracker adoption is served by keeping an existing `work.key` prefix and adding
  the scope key alongside it; reconciling long-lived local prefixes with `id`
  immutability beyond that is 0230's concern.

## Technical Notes

- Sequenced after 0220, which still reads `work.default_project_code`; this item
  renames that touchpoint.
- The Linear team key → UUID resolver already exists
  (`cli/linear-client/src/auth.rs`); this item does not change resolution, only the
  config field the scope key is read from and its ownership layer.
- The tracker-aware alias resolves a `work.`-named legacy field into a `<tracker>.*`
  slot when tracker-backed — a one-time, deprecated-only cross-section resolution,
  acceptable for a field on its way out.

## Drafting Notes

- The model separates the integration-owned scope key from the local ID prefix
  (`work.key`), each owned by its own layer and neither derived from the other. These
  are the two concepts `default_project_code` conflated; fusing them via a both-set
  error would only relocate the overload. `work.key` deliberately does not default to
  the scope key: a silent default would give local IDs the scope key's value as their
  prefix instead of an independent number sequence (`PP-0001` locally vs `PP-758`
  remotely), falsely
  implying a correspondence — so `{key}` requires an explicit `work.key`. A divergent
  `work.key` cannot mis-scope discovery — scope reads only the integration key — so
  0220 cannot recur.
- No warning on a divergent or mirrored prefix: whatever `work.key` is set to is a
  supported, explicit configuration, not a smell.
- Ownership inverted so integration skills never depend on `work.*` — chosen over a
  "work.key overrides the integration key" model, which would couple them in the
  combined deployment.
- The tracker-aware alias migrates the legacy field into the scope key, its
  load-bearing role, and materialises an explicit `work.key` where the legacy pattern
  used `{project}`, so the local ID prefix is preserved and made visible rather than
  silently defaulted.
- Key → creation-home resolution stated as an explicit deliverable because 0229's base
  scope depends on it; the resolver already exists, so this is a wiring-and-contract
  statement, not new machinery.

## References

- Parent: 0146
- Related: 0220; 0227 — accelerator config validate Command
- Blocks: 0229 — Per-Tracker Pull Scope Configuration; 0230 — Tracker-owned
  work-item ID generation
