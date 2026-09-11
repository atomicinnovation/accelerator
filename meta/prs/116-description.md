---
type: "pr-description"
id: "116"
title: "[0228] Layered Configuration Key Model"
date: "2026-09-11T09:11:55+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0228"
parent: "work-item:0228"
relates_to: ["work-item:0220", "work-item:0227", "work-item:0229", "work-item:0230"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/116"
pr_number: 116
tags: ["configuration", "work-management", "migration", "tracker"]
revision: "24c943a6c7385efc813ff8a3c23b8a3b3c866727"
repository: "accelerator"
last_updated: "2026-09-11T09:11:55+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0228] Layered Configuration Key Model

## Summary

Splits the overloaded `work.default_project_code` into two independently-owned
values: the integration-owned **scope key** (`jira.project_key` /
`linear.team_key`) that resolves the tracker's creation-home entity and
discovery scope, and the work-owned **local ID prefix** (`work.key`, spelled
`{key}` in `id_pattern`). Neither derives from the other — when `id_pattern`
references `{key}`, `work.key` is required, with no silent fallback to the scope
key. A read-time deprecation alias plus the `m0009` migration carry every
existing config across a one-release window: `work.key` ships in 1.24.0; the
alias and the `{project}` token are removed in 1.25.0.

## Changes

- **Config schema** — adds `work.key`, `jira.project_key`, and
  `linear.team_key` to the catalogue (`cli/config`); `work.key` defaults empty.
- **`{key}` pattern token** — recognised in both ID engines (the
  `corpus-adapters` regex compiler and the regex-free `corpus` scheme), with
  `{project}` kept as a deprecated synonym producing byte-identical output; a
  brace-aware `references_key` predicate is single-sourced in `corpus` and
  re-exported.
- **Shared deprecation alias** — `resolve_with_deprecated_fallback` /
  `AliasedScalar` in `cli/config` (`legacy_alias.rs`), gated on
  `work.integration`, deduped once per command, naming
  `REMOVAL_RELEASE = 1.25.0`.
- **Local ID prefix** — `resolve_scheme` reads `work.key`, requires it exactly
  when the pattern references `{key}`, and gates the scheme field so a
  bare-numeric tracker repo keeps prefix-less IDs.
- **Scope-key ownership** — Jira resolves `jira.project_key`, Linear resolves
  `linear.team_key` (catalogue `/team/key` fallback preserved); `work sync`
  discovery scope dispatches on `work.integration` through one extracted
  resolver, keeping `sync.rs` and `auth.rs` on a single resolution order.
- **Init writeback** — `init-linear` writes the discovered team key (confirmed
  overwrite, fail-safe without a TTY, `--force` for automation); `init-jira`
  reporting retargeted to `jira.project_key`.
- **`m0009` migration** — materialises the split on disk at the level each key
  is read from, pattern-conditional on `work.key`, never clobbering an already
  pinned scope key, forcing `0600` on the personal file and its backup.
- **Visualiser** — server and frontend wire vocabulary renamed to `key` in
  lock-step.
- **Docs and meta** — config and skill prose retargeted off
  `default_project_code`; the plan, codebase research, and passing
  plan-validation report are included.

## Context

- Work item: `meta/work/0228-layered-configuration-key-model.md`
- Plan: `meta/plans/2026-09-10-0228-layered-configuration-key-model.md` (done)
- Validation: `meta/validations/2026-09-10-0228-layered-configuration-key-model-validation.md` (pass)
- ADR-0044 (remote identity on `external_id`) and ADR-0047 (multi-level
  configuration) are reused unchanged.
- Blocks 0229 (per-tracker pull scope) and 0230 (tracker-owned ID generation);
  reconciles the `work.default_project_code` discovery-scope touchpoint 0220
  shipped against.

## Testing

- [x] `mise run cli:check` — green on the branch tip (format, lint, types
      across the workspace)
- [x] `mise run public-api:check` — green (the `config` and `work` snapshots
      regenerated for the new keys and the renamed variant)
- [x] `mise run frontend:check` — green (visualiser wire-vocabulary rename)
- [x] `mise run test:unit:cli` — 2938 passed, 1 skipped (the live-credential
      `tracker-contract` suite), 0 failed; `work` and `work-cli` re-verified
      green after the `MissingProject`→`MissingKey` rename
- [ ] Full `mise run` default (docs, Playwright e2e, and Python `tasks/` lanes)
      — not run in this environment; run before merge

## Notes for Reviewers

- Team-file rewrite. `m0009` rewrites team `config.md` (like `m0004`); once
  migrated, the config is unreadable by any pre-1.24.0 plugin, so a colleague
  or CI runner on the older version fails to mint IDs until they upgrade. The
  `.0009.bak` sidecar is the recovery path.
- Follow-up contract. The 1.25.0 removal must refuse to start with a
  `/accelerator:migrate` error on detecting a legacy key or a `{project}`
  pattern, rather than silently dropping recognition — recorded in the plan's
  Migration Notes so it is not inherited unguarded.
- Deprecation window is structural, not version-gated. The alias resolves
  legacy configs in memory so there is no hard-migrate gap; `m0009` is
  durability only.
- Branch scope. This stack carries one non-0228 meta commit — "Review work item
  0229 and mark ready" — that the 0228 work was based on; included deliberately
  rather than rebased out.
- Residual vocabulary. The sibling `AllocationError::ProjectUnused` still
  carries project vocabulary (its emitted error string is already
  `E_PATTERN_KEY_UNUSED`); a candidate follow-up, out of this PR's scope.
- Out of scope. No `{project}`/alias removal (that is the 1.25.0 change), no
  per-origin local prefixes (0230), and no `config validate` command (0227).
