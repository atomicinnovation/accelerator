---
type: "pr-description"
id: "27"
title: "[0168] Fold visualiser into CLI"
date: "2026-07-26T17:15:54+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "work-item:0168"
parent: "work-item:0168"
relates_to: ["work-item:0165"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/27"
pr_number: 27
tags: ["rust", "visualiser", "cli", "launcher", "corpus", "workspace"]
revision: "bd50e666663893c5f79a0a65869b6da7b076ca24"
repository: "build-system"
last_updated: "2026-07-26T17:15:54+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0168] Fold visualiser into CLI

## Summary

Folds the standalone `accelerator-visualiser` crate into the `cli/` Rust workspace. It becomes a workspace member, sheds its duplicated domain logic onto the shared `corpus`/`corpus-adapters`/`document`/`config` crates, moves its start/stop/status orchestration out of shell scripts into `accelerator visualiser …` dispatched by the unified launcher, and switches its distribution from the bespoke `launch-server.sh` + flat `checksums.json` to the signed release manifest. The outcome is one workspace, one inherited version, one dispatch path, and one signed distribution channel — with no user-facing behaviour change.

## Changes

Delivered across the plan's five phases, each independently mergeable and green:

**Phase 1 — Relocate + workspace member.** History-preserving move of `server`+`frontend` from `skills/visualisation/visualise/` to `cli/visualiser/`; the server becomes the 12th `cli/` workspace member inheriting the workspace version/edition/MSRV; every build-system, CI, and docs path is repointed and version coherence flows through workspace inheritance.

**Phase 2 — Retire onto the shared crates (parity-verified).** The server's private copies of domain logic are deleted and delegated to the shared crates: the frontmatter engine moves `serde_yml` → `serde-saphyr` (via `document`/`corpus-adapters`); doc-type/typed-ref, slug + work-item-ID, and clustering/linkage move onto `corpus`; the atomic write path moves onto `corpus_adapters::FileCorpusStore` over the shared store. `thiserror` 1 → 2 and the cargo-deny allow-list is re-pruned. Frozen golden fixtures pin byte/behaviour parity before each deletion.

**Phase 3 — Re-home orchestration.** The binary grows a clap subcommand tree (`serve`/`start`/`stop`/`status`); the shell lifecycle is ported into Rust (SIGTERM→SIGKILL escalation via `kill(pid,0)` polling, `flock(2)` serialisation, state-file management), dispatched through the launcher. The server reads `.accelerator/*.md` directly (Model 1) instead of a generated `config.json`. The Origin guard is hardened to an exact-host URI parse, and the dev circus stack is rewired onto the Model-1 `serve` path. SKILL.md now invokes `accelerator visualiser`.

**Phase 4 — Producer wiring + local-manifest verify.** `DISPATCHED_SUBBINARIES` gains `visualiser`, engaging the signed-manifest flow (stage + minisign + list). A full-symbol byte-scan guards against the `dev-frontend` insecure switch leaking into a release binary, and an automated coherence check binds the SKILL.md invocation to manifest membership. Fetch/verify/reject is proven against a local manifest fixture.

**Phase 5 — Distribution cut-over.** Deletes the retired shell surface (`visualiser.sh`, `launch-server.sh`, `stop-server.sh`, `status-server.sh`, `write-visualiser-config.sh` and their test suites), the flat `bin/checksums.json`, and the bash CLI. The checksums producer and its coherence read are removed; version coherence now spans `plugin.json` + the workspace manifest + pinned members.

**Test-robustness follow-ups.** Two flakes surfaced by the full-gate run were fixed: cli test fixtures now use self-cleaning `tempfile::TempDir` (they were pid-named dirs that collided and leaked under nextest's process-per-test model), and the visualiser E2E harness roots the server at its fixtures directory so doc API paths resolve relative and the served files are the ones the specs mutate.

## Context

- Work item: `meta/work/0168-fold-visualiser-into-cli-workspace.md`
- Plan: `meta/plans/2026-07-23-0168-fold-visualiser-into-cli-workspace.md`
- Research: `meta/research/codebase/2026-07-23-0168-fold-visualiser-into-cli-workspace.md`
- Validation: `meta/validations/2026-07-23-0168-fold-visualiser-into-cli-workspace-validation.md` (result: **pass**)
- ADRs: ADR-0045, ADR-0053, ADR-0054
- Related: work item 0165 (multi-binary distribution / signed release manifest)

## Testing

- [x] Read-only CI mirror green: `mise run check` exits 0 (format + lint + types across frontend, server, cli, build-system, scripts)
- [x] `mise run test:unit:cli` — 576 passed
- [x] `mise run test:unit:tasks` — 339 passed; `test:integration:tasks` green
- [x] `mise run test:unit:visualiser` (334) + `test:integration:visualiser` pass in isolation
- [x] `mise run test:e2e:visualiser` — 343 passed
- [x] Parity golden fixtures, write-path preservation/concurrency, Host/Origin 403, orchestration lifecycle, launcher-dispatch, and local-manifest fetch/verify (incl. tamper/reject) suites
- [x] Resolved rebased workspace compiles: `cargo check -p accelerator-visualiser` exits 0
- [ ] Full `mise run` end-to-end — green except two **pre-existing** timing-sensitive parallel-load flakes (see Notes); both pass reliably in isolation
- [ ] Manual: live `accelerator visualiser start|stop|status` from a real repo, visual spot-check of rendered views, idle self-shutdown (tracked in the validation report's manual checklist)

## Notes for Reviewers

- **Co-release invariant.** Phase 3 (SKILL.md switches to `accelerator visualiser`) and Phase 4 (producer stages/signs the `visualiser` manifest entry) form a co-release unit — `validate_dispatch_coherence` fails CI if one ships without the other, so `start` never resolves to `AssetNotFound`.
- **Rebased onto v1.24.0-pre.16.** The branch was rebased onto the latest `main`; the version-bump conflicts (server `Cargo.toml`/`Cargo.lock`, `checksums.json`) were resolved so the server inherits `pre.16` via workspace membership and the retired files are dropped.
- **The two full-gate failures are pre-existing flakes, not regressions.** They are timing-sensitive tests (one with a documented reaper/`stop` race) that pass in isolation and match this environment's known "flakes under full-run parallel load" pattern — worth hardening as a separate item.
- **Deviations from the plan letter** are catalogued in the plan's Implementation Progress log and the validation report, all within phase intent (e.g. `frontmatter.rs` presentation helpers retained after the engine swap; the "unify to one shared asset" distribution decision reshaping Phases 4–5).
- **No user-facing behaviour change** is intended — the visualiser starts, serves, and renders as before, now via `accelerator visualiser`.
