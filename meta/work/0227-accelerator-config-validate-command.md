---
type: "work-item"
id: "0227"
title: "accelerator config validate Command"
date: "2026-08-28T14:14:31+00:00"
author: "Toby Clemson"
producer: "create-work-item"
status: "draft"
kind: "story"
priority: "medium"
relates_to: ["work-item:0221", "work-item:0226"]
tags: ["config", "validation", "cli", "correctness"]
last_updated: "2026-08-29T17:07:24+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
external_id: "PP-757"
---
# 0227: accelerator config validate Command

**Kind**: Story
**Status**: Draft
**Priority**: Medium
**Author**: Toby Clemson

## Summary

As a plugin maintainer, I want an `accelerator config validate` command that
checks `.accelerator/` config on two axes — frontmatter quoting against the
canonical standard and semantic config validity — so a malformed or meaningless
configuration is caught when the config is written rather than silently ignored
until an unrelated skill trips over it. Today the config store is fully dynamic: an
unknown key or a typo'd section resolves to `Source::Unset` with no warning, and
per-key value rules fire only lazily at each consume site. Nothing validates a
config file as a whole.

## Context

This item is handed off from 0221 (canonical quoting standard for all
frontmatter), which brings config frontmatter into conformance on write (via the
shared renderer) and in one migration pass, but explicitly excludes *validating*
config from its scope. `corpus frontmatter validate` cannot cover config: config
files carry no doc-type schema (`type`/`id`/base fields), so the structural
validator rejects them as `INVALID-TYPE`.

Config validity is defined today in two disconnected places, enforced nowhere
centrally:

- The recognised-key catalogue `cli/config/src/catalogue.rs` — 55 keys across six
  groups plus `EXTRA_KEYS` (presence-only integration keys) and the 13 doc types
  — drift-tested against the bash mirror in `scripts/config-common.sh`.
- The store `cli/config/src/service.rs` is dynamic: `Key::parse` accepts any
  well-formed dotted key, and an unrecognised key resolves silently to
  `Source::Unset`. Only `work.*` emits an "unknown key" stderr note.

Per-key value rules already exist but live at the consume sites and fire only
when that key is used:

- `work.integration` enum (`catalogue::is_valid_work_integration`), enforced
  fail-closed at `cli/launcher/src/config_command/core/work.rs:29`.
- `work.id_pattern` DSL (`PatternError` rules 1-5,
  `cli/corpus-adapters/src/work_item_pattern.rs`).
- `work.default_project_code` required when the pattern carries `{project}`
  (`cli/migrate/src/migrations/m0002.rs:70`).
- `visualiser.kanban_columns` non-empty and `visualiser.idle_timeout` humantime,
  at server boot (`cli/visualiser/server/src/config.rs`).
- Credential placement — `token_cmd` refused in shared/tracked config, personal
  config refused if looser than `0600` (`cli/tracker-support/src/credentials.rs`).

The documented source of truth is `skills/config/configure/SKILL.md` (the `help`
output), enumerating recognised keys, defaults, and per-section constraints. The
command surface `cli/launcher/src/config_command/inbound/cli.rs` has no `validate`
action today; the nearest behaviour is `dump`/`view`, which only annotates a bad
`work.integration`.

## Requirements

**Command** — add an `accelerator config validate` subcommand that validates the
merged team + personal config for a project and exits non-zero on any violation,
with a clear per-violation message (key, rule, offending value).

**Frontmatter quoting** — validate that `.accelerator/config.md` (and
`config.local.md` when present) conforms to 0221's canonical quoting standard,
reusing 0221's type-driven predicate. Skip the doc-type/base-field structural
checks — config is not a doc-type artefact.

**Semantic validity** — check, at minimum:

- Unknown keys/sections against the catalogue and `EXTRA_KEYS`, with a
  per-section policy for where extra keys are legitimately allowed (e.g.
  `agents.*` warns-and-ignores today).
- `work.integration` against the known-tracker set.
- `work.id_pattern` compiles under the DSL rules, and `work.default_project_code`
  is present when the pattern carries `{project}`.
- `visualiser.kanban_columns` non-empty; `visualiser.idle_timeout` parses.
- Credential-placement rules (`token_cmd` not in shared/tracked config; personal
  config permissions).

**Reuse, not re-implement** — the command aggregates the validators that already
exist at consume sites rather than duplicating their rules, so validation and
runtime agree by construction.

**Producer-run validation** — config-writing skills (e.g. `configure`) run
`accelerator config validate` on the config file they just wrote and surface any
violation before completing; the command is also available to run on demand. This
checks every plugin user's config as they use the plugin, not only this
repository — no CI lane runs it.

## Acceptance Criteria

- [ ] Given a config with an unknown key or a typo'd section (e.g.
      `work.integraton: jira`), when `accelerator config validate` runs, then it
      reports the unrecognised key against the catalogue and exits non-zero —
      where the section does not legitimately allow extra keys.
- [ ] Given `work.integration: gitlab`, when validate runs, then it reports the
      value as outside the known-tracker set and exits non-zero.
- [ ] Given `work.id_pattern: "{project}-{number:04d}"` with no
      `work.default_project_code`, when validate runs, then it reports the
      missing project code and exits non-zero.
- [ ] Given a malformed `work.id_pattern` (e.g. `"TASK"` with no `{number}`),
      when validate runs, then it reports the DSL rule violated and exits
      non-zero.
- [ ] Given `jira.token_cmd` in the team `config.md`, when validate runs, then it
      reports the disallowed placement and exits non-zero.
- [ ] Given a `config.md` whose frontmatter violates the canonical quoting
      standard, when validate runs, then it reports the quoting violation
      (reusing 0221's predicate) and exits non-zero.
- [ ] Given a fully valid config, when validate runs, then it exits 0 with no
      violations.
- [ ] Given a skill that writes config (e.g. `configure`), when it finishes
      writing `.accelerator/config.md` or `config.local.md`, then it runs
      `accelerator config validate` on that file and surfaces any violation before
      completing.
- [ ] The command reuses existing consume-site validators rather than duplicating
      their rules (verified by a test that a rule added at a consume site is
      reflected by validate).

## Open Questions

- Unknown-key policy: does an unrecognised key fail the command, or warn? It must
  vary by section — `agents.*` and `templates.*` accept user-defined values,
  while a typo'd `work.*` key is almost certainly a mistake. Where is the line?
- Does the command centralise the scattered value rules into a reusable
  validation surface, or invoke each consumer's existing check in place? Reuse is
  preferred, but some checks (server-boot kanban/idle) currently live in the
  visualiser server, not a library the CLI can call cheaply.
- Should validate honour the executable-path-injection constraints from 0226, or
  is that a separate enforcement path?

## Dependencies

- Blocked by: 0221 (canonical quoting standard) — validate reuses its quoting
  predicate and enforces the standard it ratifies.
- Relates to: 0226 (audit repo-settable config keys for executable-path
  injection) — a security constraint validate should eventually honour.
- Blocks: none.

## Assumptions

- The recognised-key catalogue plus `EXTRA_KEYS` is the authority for "known
  key"; `skills/config/configure/SKILL.md` is its human-facing mirror. Validate
  checks against the catalogue, not a new schema.
- The existing per-key value rules are correct and complete enough to reuse;
  validate surfaces them eagerly rather than redefining them.
- Validating the merged team+personal config (not each layer in isolation) is the
  right unit, matching how config is consumed at runtime.

## Technical Notes

- No `Action::Validate` exists in
  `cli/launcher/src/config_command/inbound/cli.rs`; the command is a new action
  alongside `summary`/`detect`/`init`/`dump`.
- The scattered validators to aggregate: `work.rs:29` (integration),
  `work_item_pattern.rs` (`PatternError`), `m0002.rs:70` (project-code coupling),
  `server/src/config.rs` (kanban/idle — currently server-only), `credentials.rs`
  (token placement/permissions). Some are not yet in a crate the CLI links;
  extracting them is part of the work.
- The quoting predicate should be the same one 0221 adds to the
  renderer/validator, imported rather than re-derived, so config and corpus
  enforce byte-identical rules.

## Drafting Notes

- Parent left empty. 0136 is the Rust CLI *migration* epic; this is a new
  capability, not a migration, so it does not obviously belong there. Set a
  parent during refinement if one fits.
- Priority proposed medium: it hardens a silent-failure gap rather than
  unblocking shipping work. 0221 (high) is the prerequisite that changes on-disk
  bytes.
- Scope drawn at aggregating existing rules plus catalogue membership and
  quoting. Building a brand-new config schema language is explicitly not
  proposed.
- The two halves (frontmatter quoting, semantic validity) were named by the
  author when splitting this out of 0221; both kept in one command.
- Enforcement is producer-run, not a CI lane, at the author's direction (matching
  0221): config-writing skills validate the config they write, protecting plugin
  users' configs as well as this repository. The accepted tradeoff is that a
  hand-edited config is not checked until a skill next writes it.

## References

- Related: 0221, 0226, 0167
- Source: `skills/config/configure/SKILL.md` — documented recognised-key contract
- ADR-0033: Unified base frontmatter schema for meta/ artifacts
