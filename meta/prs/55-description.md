---
type: "pr-description"
id: "55"
title: "[0170] Work-Item Lifecycle Subdomain"
date: "2026-08-08T08:40:36+00:00"
author: "Toby Clemson"
producer: "describe-pr"
status: "complete"
work_item_id: "0170"
parent: "work-item:0170"
relates_to: ["work-item:0194"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/55"
pr_number: 55
tags: []
revision: "d54f19d8a2816513a4c11da628dc2e314c600550"
repository: "accelerator"
last_updated: "2026-08-08T08:40:36+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# [0170] Work-Item Lifecycle Subdomain

## Summary

Replaces the bash work-item lifecycle scripts with a typed, characterization-tested `accelerator work` command family — `create`, `show`, `resolve`, `diff`, and `update`, plus the `template-hints`, `canonicalise-id`, and `next-number` utility subcommands the work skills orchestrate. These commands are local-only: `--push` support and the remote sync engine are 0194's scope, split off from this story on 2026-08-05 once the combined work was found to be epic-scale for a `kind: story` item.

## Changes

- **New `work` domain crate** (`cli/work/`) implements the lifecycle operations over the shared `corpus`/`config`/`store` crates — `create`, `show`, `update` (including tag mutations), `resolve`, and `diff` (section-diff), plus the internal helpers `next_number`, `normalise`, `own_identity`, `file_dirty`, and `template_hints`. These stay private functions rather than separate subcommands, since Rust doesn't need bash's subprocess boundaries for testability.
- **New `work-adapters` crate** (`cli/work-adapters/`) supplies the ports the domain crate needs: `author` (VCS identity), `diff_shellout` (shells out to the real `diff` binary, tolerating its exit-1-is-a-difference convention), `filesystem`, and `project_remote`.
- **New `work-cli` crate** (`cli/work-cli/`) is the clap-based CLI wrapper, registered as a dispatched sub-binary under the `work` token per the checklist 0187 added. Its `--help` surface is frozen against a golden fixture (`tests/fixtures/cli_surface.golden`).
- **`corpus-adapters` absorbs the pattern-DSL port and a VCS-facts port.** `work_item_pattern.rs` completes the ID-pattern DSL port that previously lived only in bash; `corpus`'s metadata derivation now takes a `RepoFactsProbe` port, with `VcsBackedRepoFactsProbe` in `corpus-adapters` as the real implementation, decoupling the domain from a direct VCS dependency.
- **`config-adapters` consolidates `ACCELERATOR_PLUGIN_ROOT` reads** into a single call site, so every composition root (launcher, visualiser server) reads the plugin-root env var through one function rather than each doing it independently.
- **Ten bash scripts and their shared library are deleted**: `work-item-common.sh` (477 lines) plus `next-number`, `pattern`, `read-field`, `read-status`, `resolve-id`, `section-diff`, `template-field-hints`, and `update-tags`, along with `test-work-item-pattern.sh` and the 1214-line `test-work-item-scripts.sh` suite that covered them. Each was characterized before being ported — a test capturing its pre-port behaviour, covering every documented flag/argument combination and at least one error path — so the coverage gap the bash scripts had (no dedicated test suite for 10 of them) closes rather than moves. Three scripts stay for now, deferred out of this story's scope: `work-item-file-dirty.sh`, `work-item-project-remote.sh`, `work-item-normalise.sh`.
- **`create-work-item`, `update-work-item`, `list-work-items`, `refine-work-item`, `review-work-item`, `extract-work-items`, and `sync-work-items` skills repoint from the deleted bash scripts to the new CLI subcommands.** `update-work-item/SKILL.md` changes most substantially (129 lines) since it owned most of the tag/field mutation logic the new `work update` command now handles.
- **Comment-policy cleanup.** Two trailing commits consolidate the "comments are a last resort" guidance into `CLAUDE.md` and the `create-plan`/`implement-plan`/`review-plan` skill instructions, then bring every comment this branch introduced into line with it — stripping references to ADR/work-item/plan-phase numbers that go stale and comments that just restated the code beneath them, while keeping the ones documenting genuine invariants or external constraints (e.g. the `LANG=C` rationale in `normalise.rs`, the `diff` exit-code tolerance in `diff_shellout.rs`).

## Context

Split on 2026-08-05 from the original combined story (lifecycle CRUD + the `tracker` crate + the remote sync engine), following a work item review that found the two halves independently deliverable — see `meta/work/0170-work-item-subdomain-and-sync-engine.md`. The sync engine and `--push` wiring move to 0194, which will layer a `--push` flag onto this story's `create`/`update` commands once they exist; every command in this PR is local-only with zero dependency on 0194's `RemoteTracker` port.

Full research, plan, review, and validation trail: `meta/research/codebase/2026-08-06-0170-work-item-lifecycle-subdomain.md`, `meta/plans/2026-08-06-0170-work-item-lifecycle-subdomain.md`, `meta/reviews/plans/2026-08-06-0170-work-item-lifecycle-subdomain-review-1.md`, `meta/validations/2026-08-06-0170-work-item-lifecycle-subdomain-validation.md`.

## Testing

- [x] `mise run cli:check` (rustfmt, clippy) passes across the whole `cli/` workspace.
- [x] `mise run pup:check` (cargo-pup architecture rules) passes — `work` imports only `std`/`kernel::Error`/`corpus`/`crate`, and `work_adapters::filesystem` never imports `std::process`.
- [x] `cargo test -p work -p work-adapters -p corpus -p corpus-adapters --locked` — 250 passed.
- [x] `cargo test -p accelerator-work --locked` — 61 passed; with `--features bash-parity` (the real `diff` binary parity suite) — 78 passed.
- [x] `mise run lint:dispatch-coherence:check`, `lint:skill-permissions:check`, and `lint:scripts:check` all pass.
- [x] `mise run test:integration:work` passes — 108 + 6 + 21 assertions across suites, `_EXPECTED_WORK_SUITES` at the decremented floor of 5.
- [x] `mise run build-system:check` and `mise run scripts:check` pass after the comment-policy cleanup pass.
- [x] `mise run` (the full local CI mirror) — green end to end against this branch's tip (540 integration assertions in the migrate suite alone, plus every unit/lint/type-check task). The validation report had flagged one failure mid-branch, `test_bootstrap_exports_the_one_plugin_root_the_launcher_reads`, caused by a since-superseded commit (`98c17ab3f233`, the `ACCELERATOR_PLUGIN_ROOT` consolidation) rather than by 0170's own scope; a later commit on this branch (`344d8d64`, "Point the plugin-root reader test at config-adapters, not the launcher") fixed it.
- [ ] The four manual skill end-to-end runs the validation report calls out are still outstanding: driving `create-work-item` with no integration configured, `update-work-item` on a scratch item (confirming the id-immutability error surfaces the CLI's message), `update-work-item`/`list-work-items`/`create-work-item`/`extract-work-items` against a scratch item (confirming no residual shell-out to a deleted script), and `sync-work-items`' dirty-check guard against a deliberately-dirtied local file.

## Notes for Reviewers

- **Internal-vs-subcommand boundary is a judgment call, not a spike-confirmed one.** `pattern`, `template-field-hints`, `file-dirty`, `project-remote`, and `normalise` stay as private functions rather than becoming their own subcommands — recorded as an open question on the work item in case the bash-era script boundary turns out to matter for a reason not visible from a script's header comment alone.
- **`work_item_id: PP-191`'s remote counterpart still reflects the pre-split title/scope.** Reconciling it needs `accelerator work update --push`, which doesn't exist until 0194 wires it onto this story's `update` command — tracked as a follow-up on the work item, not blocking this PR.
- Worth a close look at `cli/corpus-adapters/src/metadata.rs`'s new `RepoFactsProbe` port and `VcsBackedRepoFactsProbe` implementation — it's the one piece of this PR that changes an existing crate's dependency direction rather than adding new surface area.
- References to the deleted script names remain, deliberately, in `skills/work/*/evals/benchmark.json` (grading-criteria prose describing historical skill behaviour) and in the characterization golden files' own header comments — inert historical record, not live shell-outs.
