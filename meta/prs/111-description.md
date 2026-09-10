---
type: "pr-description"
id: "111"
title: "[0184] Refuse template resolution on a wrong plugin root"
date: "2026-09-10T00:47:43+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0184"
parent: "work-item:0184"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/111"
pr_number: 111
tags: ["cli", "config", "templates", "plugin-root", "visualiser"]
revision: "6b66bc059057ecb330226b459ae7d9e2b65edd8e"
repository: "accelerator"
last_updated: "2026-09-10T00:47:43+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0184] Refuse template resolution on a wrong plugin root

## Summary

Extends 0182's *absent*-root refusal to the *wrong*-root case: a present
plugin root that carries no `templates/` directory is not an Accelerator
installation, so every template command and the visualiser compose path now
refuse with a dedicated diagnostic instead of degrading to a silent empty
answer or a false template-not-found. The refusal names the offending root,
the invalid `templates/` layout, and `ACCELERATOR_PLUGIN_ROOT`, and fails
closed even under `--fail-safe`.

## Changes

- **New error variant** — `ConfigError::PluginRootNotAnInstallation { path }`
  (`cli/config/src/error.rs`), classified `is_refusal() == true` so it fails
  closed identically to `PluginRootUnavailable`, with a message that names the
  path and states the root is not an installation (distinct from the
  absent-root "unknown, set the variable" message).
- **Shared installation-detection helper** — `require_templates_dir`
  (`cli/config-adapters/src/store.rs`) validates `templates/` with a single
  `fs::metadata` probe and becomes the one choke point for `template_names`,
  `plugin_template_path`, `plugin_default`, `eject`, and `resolve_template`'s
  plugin-default tier.
- **Structural shapes fail closed** — a missing `templates/`, a `templates`
  that is a file, and a root that is itself a file all refuse; a genuinely
  *unreadable* fault (permission, filesystem loop) stays the degradable `Io`
  and does not name the variable.
- **Preserved behaviour** — a resolving user override still renders at exit 0
  under a wrong root (override tiers precede the plugin-default check), a valid
  installation missing one template still reports a template-not-found naming
  the template, and the root-independent config families are untouched.
- **Compose boundary** — no production change; the existing `?` at
  `compose.rs:157` propagates the refusal into `ComposeError`, pinned by a new
  contract test.
- **Fixtures** — regenerated the config `public-api.txt` for the new public
  variant and seeded `templates/` in the shutdown-test plugin-root fixture,
  which compose now correctly refuses without.

## Context

- Work item: `meta/work/0184-template-enumeration-swallows-a-wrong-plugin-root.md`
- Plan: `meta/plans/2026-09-09-0184-template-resolution-wrong-plugin-root.md`
- Research: `meta/research/codebase/2026-09-08-0184-template-resolution-wrong-plugin-root.md`
- Validation: `meta/validations/2026-09-09-0184-template-resolution-wrong-plugin-root-validation.md`
- Builds on 0182 (CLI derives plugin root from its own location).

## Testing

- [x] Error-crate unit tests — `cargo test -p config` (new variant Display + `is_refusal`)
- [x] Store unit tests — `cargo test -p config-adapters` (10 new tests: three structural refusals, the `Io` arm, the override hoisting crux, genuine-not-found preservation)
- [x] Launcher integration — `cargo test -p accelerator --test config_read` (152 passed, all six commands on a wrong root)
- [x] Compose contract — `cargo test -p accelerator-visualiser --test compose_contract` (wrong root refuses to compose)
- [x] Format + lint — `mise run cli:check` (rustfmt, clippy, store-duplication)
- [ ] Full local CI mirror — `mise run` recorded green in the work item after the docs-site advisory rebase; not re-run in this validation session

## Notes for Reviewers

- **AC8 semantics moved.** The work item's AC8 originally treated a
  `templates`-is-a-file root as the representative `Io` fault; this change
  reclassifies that shape as a structural refusal and reserves `Io` for
  genuinely unreadable faults. AC8 was revised and AC13 added for the two extra
  structural shapes — both reflected in the work item.
- **Deliberate residual duplication.** `compose.rs:155` still reconstructs
  `plugin_root.join("templates")` to build each tier's absolute default path;
  the validity gate is centralised, but relocating the per-name path
  construction behind a store accessor is a wider compose-tier refactor left
  out of scope. The `store-duplication` lint passes.
- Worth focusing on: the ordering invariant that keeps a resolving override
  ahead of the root check (`resolve_template`), and the `NotFound |
  NotADirectory` collapse in `require_templates_dir`.
