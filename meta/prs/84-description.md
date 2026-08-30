---
type: "pr-description"
id: "84"
title: "Empty scripts/ and retire shell tooling and CI guards"
date: "2026-08-29T08:15:27+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0174"
parent: "work-item:0174"
pr_url: "https://github.com/atomicinnovation/accelerator/pull/84"
pr_number: 84
tags: ["shell", "tooling", "ci", "cleanup", "scripts"]
revision: "a9cf9fb6ad2ada3ef8f661ad63fdc69ed41ec9de"
repository: "accelerator"
last_updated: "2026-08-29T08:15:27+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# Empty scripts/ and retire shell tooling and CI guards

## Summary

The shell-to-Rust migration is behaviourally complete, so this empties `scripts/` to the two-file thin-shell floor and retires the build-system and CI machinery that existed only to police the vanished bash library. What remains of the shell surface — the launcher bootstrap `bin/accelerator` and the hook wrapper `hooks/launcher-link-refresh.sh` — is now guarded by a Python bashisms task plus shfmt and ShellCheck over an explicit two-file list, not a whole-tree walk.

## Changes

Delivered as the ten phases of the 0174 plan plus follow-up refinements, each an independently green commit:

- **Jira external-id cutover.** New `accelerator work link-external-id` subcommand (a promoted free function in `sync_author.rs`); the Jira create-issue SKILL.md writeback repoints off the retired `config-common.sh` chain onto it.
- **Config source-chain deletion.** Removed `config-common.sh` / `vcs-common.sh` / `config-defaults.sh` / `atomic-common.sh` and every drift-oracle test that read them, rescoping the survivors to their pure-Rust cases.
- **Data-file relocations.** `templates-schema.tsv` moved into the `corpus` crate and `extract-work-items-cue-phrases.txt` into the `design` crate, each `include_str!` repointed in the same commit.
- **Bash-oracle retirements.** Deleted `doc-type-inference.sh` and the `linkage-type-pairs.tsv` oracle, backfilling native `doc_type.rs` tests for the tie-break and interior-segment cases the bash oracle uniquely covered.
- **Orphan library sweep.** Removed the residual libraries (`log-common.sh`, `fs-common.sh`, `hash-common.sh`, `doc-type-table.sh`, `accelerator-scaffold.sh`) and their fixtures.
- **Nine-guard Python port.** The authoring/evals guards now run as pytest under `tests/unit/tasks/`, with the validator-driven conformance guard homed in a new launcher-provisioning `test:integration:conformance` lane.
- **Hooks floor to zero.** Ported the two non-detection guards from `test-vcs-detect.sh` into `tests/integration/hooks/`; relocated the retained VCS goldens into `cli/vcs-test-support`.
- **Bashisms denylist → Python.** Reimplemented `lint-bashisms.sh` as an in-process Python scan over the survivor list, preserving the eight-pattern denylist and fail-closed behaviour.
- **Final retirement and rescope.** Replaced `shell_sources()` with the `SURVIVING_SHELL_SOURCES` constant, removed the exec-bit invariant, `SHELL_LIBRARIES`, and the suite floors, retired the `check-scripts` CI job (re-homed as a step in `check-build-system`), and documented the two survivors in `tasks/README.md`.

## Context

- Work item: `meta/work/0174-retire-shell-tooling-and-ci-guards.md` (Jira `PP-195`)
- Plan: `meta/plans/2026-08-28-0174-empty-scripts-retire-shell-tooling.md`
- Validation: `meta/validations/2026-08-28-0174-empty-scripts-retire-shell-tooling-validation.md` (result: pass)
- Parent epic: `meta/work/0136-migrate-shell-scripts-to-rust-cli.md`
- ADRs: ADR-0048 (four-toolchain split), ADR-0049 (bash-3.2 floor)

## Testing

- [x] `mise run scripts:check` — shfmt + ShellCheck + Python bashisms over the two survivors
- [x] `mise run build-system:check` — ruff + pyrefly + actionlint + dispatch-coherence
- [x] `mise run test:unit:tasks` — 2771 passed (ported guards + bashisms scanner)
- [x] `mise run test:integration:conformance` — 27 passed (replaces the retired `config` lane)
- [x] `mise run test:integration:hooks` — 24 passed (both ported VCS-detect guards)
- [x] `cargo test --all-features` over the six touched crates — all suites green, no warnings-as-errors breakage
- [ ] Full bare `mise run` default (frontend + docs/Chromium + repeated Rust compiles) — not re-run locally; relies on per-phase greenness and branch CI

## Notes for Reviewers

- **Per-commit greenness is the risk model.** Each phase drains the two lockstep couplings (the exec-bit `SHELL_LIBRARIES` frozenset and the config-suite floor) in one commit; the granularity is deliberate, so review the phase boundaries rather than only the net diff.
- **One intentional residual.** `tasks/measure.py`'s `RECOVERED_FILES` still names `scripts/vcs-common.sh` — it recovers that file from a pinned baseline commit as the recovered guard's runtime dependency, not from the live tree. Removing it would break `recover_baseline`.
- ⚠️ **Branch-protection handoff at merge (out-of-repo).** This PR deletes the "Check scripts" status. Its required-check must be dropped from the branch-protection ruleset and "Check build system" confirmed required (it now carries the shell lane), or PRs will hang waiting on a status no job reports.
- **Untracks a gitignored artefact.** This branch deletes `test-results/.last-run.json`, which was tracked on `main` despite matching the `**/test-results/` ignore rule; the deletion brings the tree in line with the ignore.
