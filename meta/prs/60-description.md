---
type: pr-description
id: "60"
title: "[0197] Accelerator collaboration PR helper CLI"
date: "2026-08-09T22:28:55+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0197"
parent: "work-item:0197"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/60"
pr_number: 60
tags: []
revision: "244bc6cf7a3e874e86b13e6f95a19f0ecdeaec0c"
repository: "accelerator"
last_updated: "2026-08-09T22:28:55+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# [0197] Accelerator collaboration PR helper CLI

## Summary

Migrates the two PR-helper bash scripts (`pr-base-repo.sh`,
`pr-update-body.sh`) into a new `accelerator-collaboration` sub-binary that
calls the GitHub REST API in-process via `octocrab`, removing the runtime
dependency on an installed and authenticated `gh` CLI for the
`review-pr`, `respond-to-pr`, and `describe-pr` skills. This is part of the
broader shell-to-Rust migration epic (work-item:0136).

## Changes

- **New `collaboration` domain crate** (`cli/collaboration`) — owns
  fork-aware base-repo resolution (`resolve_origin_owner_repo`), the
  `OwnerRepo` type, the `RemoteUrlRecognizer` port, and body-update
  orchestration, kept forge-agnostic (`ForgeApiError`, not
  `GitHubApiError`).
- **New `github` crate** (`cli/github`) — an `octocrab`-backed
  `GitHubRemoteUrlRecognizer` and `OctocrabClient`, plus a hand-rolled mock
  HTTP server used for characterization tests (auth header assertions,
  redirect rejection).
- **New `collaboration-cli` sub-binary** (`cli/collaboration-cli`,
  package `accelerator-collaboration`) — dispatched via
  `accelerator collaboration pr base-repo` / `pr update-body`, wired
  through every item of the sub-binary registration checklist (`.gitignore`,
  `tasks/manifest.py`, `tasks/build.py`, `tasks/shared/paths.py`,
  `tests/integration/tasks/test_github.py`).
- **`origin`-remote parsing added to `vcs`** (`cli/vcs/src/origin_remote.rs`)
  — a new `OriginRemote` port for reading the raw remote URL; URL-form
  parsing itself stays in `collaboration`/`github` to keep `vcs`
  forge-agnostic.
- **`github.token` / `github.token_cmd` config keys** added to
  `cli/config/src/catalogue.rs`'s `EXTRA_KEYS` (mirroring the existing
  `jira`/`linear` pairing) with a bash mirror in
  `scripts/config-defaults.sh`.
- **Env-first credential resolution** (`cli/collaboration-cli/src/auth.rs`):
  `GH_TOKEN` → `GITHUB_TOKEN` → `github.token` config → `github.token_cmd`,
  matching `gh`'s own documented env-var precedence and the existing
  `jira`/`linear` resolvers.
- **Personal-config permission/symlink enforcement** moved into
  `config-adapters`' `FileConfigStore` (`cli/config-adapters/src/store.rs`)
  rather than being `collaboration`-specific, so every `Level::Personal`
  config read is protected, not just `github.token`.
- **Skill call sites repointed**: `review-pr`, `respond-to-pr`, and
  `describe-pr` now invoke `accelerator collaboration pr …` instead of the
  bash scripts, with `allowed-tools` updated to match.
- **Legacy scripts and suites removed**: `skills/github/scripts/pr-base-repo.sh`,
  `skills/github/describe-pr/scripts/pr-update-body.sh`, and their three
  PATH-stubbed-`gh` bash test suites are deleted; `_EXPECTED_GITHUB_SUITES`
  in `tasks/test/integration.py` drops from 3 to 0.
- **Documentation**: new `docs-site/src/content/docs/collaboration.md` page,
  plus updates to the root README, `configuration.md`,
  `configuration-cookbook.md`, and `review-a-pr.mdx`.

## Context

Implements work-item:0197, split out of the abandoned work-item:0173 to
keep the corpus/design/collaboration sub-binary migrations independently
deliverable. Plan: `meta/plans/2026-08-08-0197-accelerator-collaboration-pr-helper-cli.md`.
Validated against the implementation with three deliberate, coherently-executed
deviations from the plan's code sketch — full detail in
`meta/validations/2026-08-08-0197-accelerator-collaboration-pr-helper-cli-validation.md`:

- Origin-remote URL *parsing* lives in `collaboration`/`github`, not `vcs`
  as originally sketched (`vcs` only exposes the raw-remote-read port).
- No separate `collaboration-adapters` crate — `github` plays that role
  directly, since every concern in it is GitHub-specific.
- The CLI nests both subcommands under `Command::Pr { action: PrAction }`
  (`accelerator collaboration pr base-repo` / `pr update-body`) rather
  than the plan's flat `base-repo`/`update-body` variants.

## Testing

- [x] `mise run cli:check` — passes
- [x] `mise run test:unit:cli` — passes (1126 tests; 261 specific to this
      change's crates — `collaboration`, `github`, `accelerator-collaboration`,
      `vcs`, `vcs-adapters`, `config`, `config-adapters` — including 17
      characterization tests in `accelerator-collaboration` covering all 8
      base-repo-resolution branches, all 5 body-update branches, and all 4
      supported remote-URL forms)
- [x] `mise run build-system:check` — passes
- [x] `mise run test:integration:github` — passes (0 shell suites
      discovered, floor is 0)
- [x] `mise run check` — passes (format, lint, and types across all four
      components, including `cargo-pup`'s whole-crate import rule and
      `lint:dispatch-coherence:check`)

Verified via `meta/validations/2026-08-08-0197-accelerator-collaboration-pr-helper-cli-validation.md`,
recorded against this branch's tip.

## Notes for Reviewers

- The base-repo resolution is a deliberate two-call design
  (`GET /repos/{owner}/{repo}` for the fork-parent check, then
  `GET /repos/{base_owner}/{base_repo}/pulls/{pull_number}` to confirm the
  PR exists at the resolved base) rather than a single call — the
  Technical Notes section of the work item explains why a single
  `.../pulls/{pull_number}` call cannot recover a different base repo than
  the one queried.
- `main.rs`'s `BlockingGitHubClient` enters a fresh Tokio runtime per call
  (no nested `block_on`s) to satisfy `tower::Buffer`'s `tokio::spawn`
  requirement inside `octocrab`'s client construction — worth a look if
  touching the CLI's async boundary.
- The plan's deviations (see Context) are coherent but undocumented in the
  plan text itself; flagging here so reviewers aren't surprised by the `pr`
  subcommand nesting or the `github` crate standing in for
  `collaboration-adapters`.
