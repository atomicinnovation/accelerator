---
type: pr-description
id: "59"
title: "[0172] Migrate migrations to Rust CLI"
date: "2026-08-10T00:17:55+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0172"
parent: "work-item:0172"
relates_to: ["work-item:0202"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/59"
pr_number: 59
tags: []
revision: "9e1a74739b3bcbd2a6dfc9e94f72260d637afc97"
repository: "accelerator"
last_updated: "2026-08-10T00:17:55+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# [0172] Migrate migrations to Rust CLI

## Summary

Ports `skills/config/migrate/` — the bash meta-directory migration engine (a 687-line driver, a 984-line FIFO/fd interactive runner, a 688-line author-facing harness, a 169-line wire protocol, three awk helpers, and 7 numbered migrations totalling 2,632 lines) — into a native Rust sub-binary, `accelerator-migrate`. Migrations now run in-process rather than as forked bash children, so the FIFO/fd IPC, the 30s watchdog, and the dual hand-rolled JSON escaper (shell writer, awk reader) are retired outright rather than ported.

## Changes

- **Three new crates**: `migrate` (domain — registry, the `Migration`/`InteractiveMigration` traits, migrations 0001-0007, the ledger, manifest, list/decisions-file logic), `migrate-adapters` (infrastructure — JSON session log, run lock, decisions-file/TTY decision sources, dirty-path scanner, corpus index, merge-move), and `migrate-cli` (the `accelerator-migrate` binary — CLI parsing, the discoverability hook, rendering).
- All 7 migrations ported and verified byte-for-byte against captured bash goldens. Migration 0007 (`unify-meta-corpus-frontmatter`) is the largest, with its own submodules for frontmatter rewrite/merge/prepass/fence/quote/schema handling.
- The interactive protocol is now an in-process accept/edit/skip loop against a `Transformation` struct, replacing the wire-protocol handshake; a JSON session log supports guarded resume across runs.
- The dirty-tree pre-flight moved in-process, backed by a new `dirty_paths` module in `vcs-adapters` (git via `gix` status, jj via a real snapshot-then-diff).
- `accelerator-migrate` registered end-to-end as a dispatched sub-binary: workspace members, cargo-pup import rules, `DISPATCHED_SUBBINARIES`, and the manifest/release/upload plumbing.
- Retirement cutover: deleted `skills/config/migrate/{scripts,migrations}/*` (driver, harness, wire protocol, awk helpers, all 7 bash migrations), `hooks/migrate-discoverability.sh` and its test, `scripts/{interactive-harness.sh,interactive-protocol.sh,jsonl-common.sh}` and their tests, the `test:integration:migrate` mise task, and every bash test-fixture underneath — 126 files removed in total, superseded by `cli/migrate-cli/tests/fixtures/` in the new Rust golden format.
- `skills/config/migrate/SKILL.md` rewritten for the Rust contract: the `Bash` permission narrows from unrestricted to `Bash(${CLAUDE_PLUGIN_ROOT}/bin/accelerator migrate *)`, and the header-marker/harness-function API reference is replaced with the `MigrationContext` trait surface. `docs-site/src/content/docs/migrations.md` updated to match.
- Suite-coverage closure: 1,010 assertions from the retiring bash suites were inventoried, and 79/80 `test-migrate.sh` gaps plus all 17 `test-migrate-0007.sh` gaps were closed with new Rust tests rather than a mechanical 1:1 rewrite (disclosed deviation — see `meta/inventories/0172-suite-audit.md`).
- Work item `0172` transitioned to `done`; follow-up `work-item:0202` filed to reconcile the migration-engine ADRs (which still describe the retired bash wire protocol) against the shipped Rust design.

## Context

Implements `work-item:0172` (Migration Engine Subdomain), planned in `meta/plans/2026-08-07-0172-migration-engine-subdomain.md`, researched in `meta/research/codebase/2026-08-06-0172-migration-engine-implementation-research.md`, reviewed in `meta/reviews/plans/2026-08-07-0172-migration-engine-subdomain-review-1.md`, and re-validated (result: pass) in `meta/validations/2026-08-07-0172-migration-engine-subdomain-validation.md`.

Built on crate foundations landed in prior work: `config`/`config-adapters` (0178), `corpus`/`corpus-adapters`/`document`/`vcs`/`vcs-adapters` (0179), the atomic-store primitives (0180), the standalone `cli/store` crate (0167), the bootstrap/fetch-verify-cache path (0164/0169), and the sub-binary registration checklist (0187).

Relates to `work-item:0202`, filed as this work's own follow-up to reconcile the migration-engine ADRs against the Rust port.

## Testing

- [x] `mise run check` — the full CI-equivalent gate (format, lint, and type-checks across frontend/server/cli/build-system/scripts, plus cargo-pup) — exits 0.
- [x] `mise run test:unit:cli` — 1487/1487 tests pass, including every `migrate`/`migrate-adapters`/`migrate-cli` suite and the byte-for-byte bash-golden parity tests (`migration_0001`…`migration_0007`, `list_and_decisions_file`, `dirty_tree_preflight`, `discoverability_hook`, `full_registry_e2e`).
- [x] Rebased onto latest `main` (past the `collaboration`/`github` PR #60 merge) and re-verified. One semantic (non-textual) merge conflict surfaced: two `migrate-cli` test fixtures wrote personal-level config files without `chmod 0600`, tripping a permission guard `main` had independently added (`store::require_owner_only_permissions`). Fixed by setting the fixture mode to match what a real `accelerator config set --local` write produces.
- [ ] Not verified this session: the Linux CI lane (only run locally on darwin).

## Notes for Reviewers

- This is a large stack (~46 commits; 472 files changed — 264 added, 126 deleted, 49 modified, 33 renamed). History is organised so each commit reviews independently: crate scaffold → engine → mechanical migrations 0001-0006 → interactive engine → migration 0007 → coverage-audit closure → retirement cutover.
- `work-item:0172` itself discloses 5 of 58 acceptance criteria left honestly unticked rather than marked complete by assertion — see the work item's own `last_updated_note` for the list.
- `work-item:0202` (ADR reconciliation) is filed but still in `draft` — the ADRs describing the old bash wire protocol are not yet updated to match this port. Flagging so it isn't assumed done as part of this PR.
- The suite-coverage closure (Phase 9) used a 4-agent audit against the assertion inventory rather than a mechanical 1:1 port of every deleted bash test; see `meta/inventories/0172-suite-audit.md` for the full accounting, including the one `test-migrate.sh` gap left open.
