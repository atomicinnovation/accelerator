---
type: pr-description
id: "51"
title: "0169: VCS subdomain and hooks migration"
date: "2026-08-06T00:59:18+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0169"
parent: "work-item:0169"
relates_to: ["work-item:0125", "work-item:0172", "work-item:0183", "work-item:0189", "work-item:0192", "work-item:0193", "work-item:0198"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/51"
pr_number: 51
tags: [rust, vcs, hooks, migration]
revision: "e3c062d0e5f507be342f3c28f8f0565adcf154fb"
repository: "accelerator"
last_updated: "2026-08-06T00:59:18+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# 0169: VCS Subdomain and Hooks Migration

## Summary

Implements ADR-0048 for the VCS subsystem: builds `accelerator-vcs`, a new dispatched sub-binary providing `vcs detect|status|log|guard` over the 0188 library-backed adapters, migrates the two VCS hooks and the `config-detect.sh` wrapper's registration from shell into the CLI, and repoints `skills/vcs/commit` at the new subcommands so the shell VCS surface (`hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`, `hooks/config-detect.sh`, `scripts/vcs-status.sh`, `scripts/vcs-log.sh`) can retire.

## Changes

- **Checkout classification** (`cli/vcs`, `cli/vcs-adapters`): moves `WorktreeFacts`/`JjWorkspaceRole`/`JjRepositoryFacts`/`DualRoots` into the `vcs` domain crate and adds a `classify()` cascade producing the shell's seven-arm taxonomy (`main`, `jj-secondary`, `git-worktree`, `colocated`, `nested-jj-in-git`, `nested-git-in-jj`, `none`), verified against the existing 34-fixture matrix.
- **Shared hook envelope** (`cli/kernel/src/hooks.rs`): a new module carrying every JSON shape both `config summary --format=hook` and the new `vcs` subcommands need — SessionStart, the PreToolUse `permissionDecision` deny shape, the bare `systemMessage`-only warn shape, and the adapter-failure shape.
- **Launcher fail-safe for external dispatch** (`cli/launcher`): closes a gap where a sub-binary resolution failure under `--fail-safe` still exited non-zero; adds an allowlisted swallow (`Failed`-class errors only, never `Refusal`/integrity failures) and reclassifies four `ResolutionError` variants from `Failed` to `Refusal` so a corrupted or improperly-signed sub-binary is never silently swallowed alongside an ordinary network hiccup — this reclassification is shared launcher code, so it also changes `accelerator visualiser`'s exit code from 1 to 2 on the same failure class (documented as an intentional fix, not a `vcs`-only side effect).
- **Cache-root probe deferred to the write path** (`cli/launcher`): splits the cache-root resolver so the write-chmod-exec probe (~132ms) only runs on a cache miss, not on every external-subcommand dispatch — a warm `accelerator vcs guard` no longer pays it.
- **`accelerator-vcs` crate and subcommands** (`cli/vcs-cli`): `vcs detect` (structured-only by default, full reference text behind `--descriptive`), `vcs status`/`vcs log` (subprocess-backed, matching the shell's `jj status`/`git diff --cached --stat`/`jj log`/`git log` exactly), and `vcs guard` (a quote-aware compound-command splitter, the 13-subcommand blocklist, jj-equivalent suggestions, and the two-shape deny/warn envelope).
- **Sub-binary registration**: `vcs` is added to `DISPATCHED_SUBBINARIES`/`_SUBBINARY_MANIFESTS`/the cli workspace/`.gitignore`/`_CLI_RELEASE_BINARIES` per 0187's checklist; `skills/vcs/commit` is repointed at the new subcommands and its broad `scripts/*` permission narrowed to the `vcs *` subcommand.
- **`hooks.json` rewrite and shell deletion**: SessionStart and PreToolUse now register three verbatim `accelerator vcs ...`/`accelerator config ...` command strings instead of five shell scripts; `hooks/vcs-detect.sh`, `hooks/vcs-guard.sh`, `hooks/config-detect.sh`, `scripts/vcs-status.sh`, and `scripts/vcs-log.sh` are deleted; the 42-case shell parity gate is repointed onto the compiled binary and its surviving in-process cases move to a new `scripts/test-vcs-common.sh`.
- **Fixture and golden capture** (`hooks/test-fixtures/`): a shared `masks.toml` (loaded by both a Rust and a Python differential test), status/log goldens for 10 checkout states, a 138-row guard decision table, and detect fixtures for the two declared behavioural departures.
- **Hand-offs**: dated notes appended to 0125, 0172, 0183, and 0189; two new follow-up work items created — 0192 (the `scripts/vcs-common.sh` residue and `hooks/launcher-link-refresh.sh`) and 0193 (the `log`/`diff` blocklist-membership decision) — plus 0198 (deferring `vcs status`/`vcs log` off subprocess execution onto the library adapters, opened after landing).

### Four declared behavioural departures from the shell

(all tested as the new behaviour, not ported as bugs)

1. The PreToolUse envelope moves to the `permissionDecision` shape (the shell never actually reached Claude Code — it nested the warning where the hook schema has no field).
2. A colocated checkout whose `.git` is a *file* (worktree/submodule) is now correctly classified `colocated` instead of misread as pure-jj — falls out of using the library-backed queries (`gix::discover`) instead of the shell's blind `-d "$REPO_ROOT/.git"` test.
3. `vcs detect`'s default output narrows to structured-only (the boundary block, or nothing); the shell's always-on "VCS Command Reference" cheat-sheet moves behind a new `--descriptive` flag, which the registered SessionStart command passes so the user-visible transcript is unchanged.
4. `vcs guard`'s compound-command splitter is quote-aware, fixing a shell defect where `git commit -m "build && test"` was wrongly split inside the quoted string.

## Context

- Work item: `meta/work/0169-vcs-subdomain-and-hooks-migration.md`
- Plan: `meta/plans/2026-08-05-0169-vcs-subdomain-and-hooks-migration.md` (all 10 phases implemented; status `done`)
- Validation: `meta/validations/2026-08-05-0169-vcs-subdomain-and-hooks-migration-validation.md` — result `pass`
- ADR-0048 (hook logic lives in the CLI), ADR-0053 (thin CLI over a hexagonal ports-and-adapters core)

## Testing

- [x] `mise run` (the full local CI mirror: build, format-fix, lint, type-check, deny/pup static checks, and the entire test suite including e2e) passes end to end — confirmed 2026-08-06.
- [x] Independent verification of all 10 plan phases against the code on disk (four parallel codebase-analyser passes) — no missing or contradicted items.
- [x] The 138-row `vcs guard` decision table and the 42-case shell parity gate pass through the compiled `accelerator-vcs` binary.
- [x] Fail-open fault injection (corrupt repository, release-host unreachable, manifest-missing-entry) exits 0 with no blocking envelope in every case.
- [ ] **Deferred, pending an actual release**: end-to-end Claude Code floor check against an installed plugin's `hooks.json`, and the `G ≤ 1.1 × B` warm-call latency measurement against a real, non-`ACCELERATOR_VCS_BIN`-overridden dispatch — both require a published manifest listing `accelerator-vcs`, recorded as open in the validation report and the work item's own Acceptance Criteria.

## Notes for Reviewers

- This PR is large (98 files) because it is the full 0169 story end to end, not a single phase — the plan and validation report above are the fastest way to navigate it; each phase is also its own commit.
- Three acceptance criteria remain deliberately unchecked on the work item, all gated on the same external prerequisite (an epic-0136 release publishing `accelerator-vcs`): the musl `_assert_static_elf` check (only exercised by `build:cli:cross-compile`, not `mise run`), the release-precedes-rewrite ordering criterion, and the warm-call latency measurement. None are code gaps — see the validation report for the full breakdown.
- The `ResolutionError`→`kernel::Error` integrity/availability reclassification (Phase 5) is shared launcher code and deliberately changes `accelerator visualiser`'s exit code from 1 to 2 on a checksum/signature/version-mismatch dispatch failure — flagged in the plan's Migration Notes as intentional, worth a second look only if anything depends on that specific exit code.
- `hooks.json`'s rewrite is safe to merge ahead of the release cut (a missing `accelerator-vcs` manifest entry degrades gracefully via the fail-safe swallow), but per Sequencing Constraint 4 it should not be treated as fully deployed until that release ships — flagged as an owner action in the work item's Dependencies.
