---
type: pr-description
id: "26"
title: "[0167] Config command and invocation contract migration"
date: "2026-07-23T01:00:42+00:00"
author: Toby Clemson
producer: describe-pr
status: complete
work_item_id: "0167"
parent: "work-item:0167"
relates_to: ["work-item:0166", "work-item:0106", "work-item:0107", "work-item:0169", "work-item:0173", "work-item:0178"]
pr_url: "https://github.com/atomicinnovation/accelerator/pull/26"
pr_number: 26
tags: [rust, config, cli, skills, invocation-contract, allowed-tools, hooks, store, migration]
revision: "4af9f104c3153a6801518e43a735c6177d16d47c"
repository: "build-system"
last_updated: "2026-07-23T01:00:42+00:00"
last_updated_by: Toby Clemson
schema_version: 1
---

# [0167] Config command and invocation contract migration

## Summary

Wires the full `accelerator config` command into the launcher over the shared `config`/`corpus` crates and cuts the plugin's invocation contract over to it: every config-cluster call site in a SKILL.md moves from a bare `${CLAUDE_PLUGIN_ROOT}/scripts/config-*.sh` path to an `accelerator config …` call, with its `allowed-tools` rules rewritten in lockstep, then the 20-script bash removal set and its shell suites are deleted. This is the highest-blast-radius story in the Rust CLI migration epic (0136) and the first production exercise of the 0164 bootstrap path — `bin/accelerator` went from referenced by zero skills and hooks to backing the entire config surface.

## Changes

**New `accelerator config` command (Rust)**

- New `config_command` hexagon in the launcher (`inbound/` → `core/` → `render/`) exposing the read families — scalar `get`, `agents`, `context`, `instructions`, `paths`, `dump`, `review`, `summary`, `template` — plus the net-new write path: `config set`, `config templates eject/diff/reset`, and `config init`. Behaviour is pinned by golden fixtures under `cli/launcher/tests/fixtures/` and exercised by `config_read.rs`.
- Subcommand-scoped refusals map to a dedicated exit-code contract via `kernel::Error::Refusal` (exit 2); `config set` refuses a non-mapping root and a symlinked target and leaves the file byte-identical.
- `config --explain` and a review lens catalogue sourced from `config-defaults.sh` round out the surface.

**Shared `store` crate**

- New `cli/store` crate extracts `atomic_write` (with `WriteError`, `NewFileMode`, `WriteBounds`, and a shared `current_umask()` hoist) as a permitted-root-checked primitive. `config-adapters` and `corpus-adapters` writes now route through it; a `store-duplication:check` lint plus a cargo-pup import rule guard against reintroducing duplicate implementations.

**Invocation-contract cutover**

- 247 `!`-preprocessor call sites across 46 SKILL.md files, 14 non-`!` sites, 28 shell consumers, and 7 migration scripts repointed from bare config-script paths to `accelerator config`, with the single `Bash(${CLAUDE_PLUGIN_ROOT}/scripts/config-*)` `allowed-tools` glob rewritten across 35 frontmatter blocks. A `check-call-site-migration.sh` / `check-skill-permissions.sh` gate (migrated to Python invoke tasks) enforces the cutover.

**Hook + removal-set cleanup**

- `config-detect` re-homed onto the bootstrap path (thin `hooks/config-detect.sh` wrapper retained pending 0169's live-SessionStart arg-splitting settlement); `vcs-detect` kept at hook index 0.
- Removal set deleted: 20 bash scripts (18 `config-*.sh` + `config-read-agent-name.sh` + `init.sh`), the 6,289-line `test-config.sh` suite, and the transitional shims. `config-common.sh`, `config-defaults.sh`, and `config-read-browser-executor.sh` survive by design.
- The config parity suites were repointed at the compiled launcher as a parity gate before deletion; the differential shell-out oracle was retired.

## Context

- Implements work item **0167** (parent of this PR; epic **0136**, Rust CLI migration). Backed by ADR-0047 (CLI as native config reader, `config get/set`) and ADR-0045 (`configure` skill as the skills-vs-CLI division).
- Also records the downstream ripples on **0106/0107** (bare-path invocation), **0166** (store crate carved out here rather than in 0180), **0169** (inherits the SessionStart envelope), and **0173/0174** (dependency edges).
- Full paper trail included in the diff: codebase research, the implementation plan and a follow-up refactoring plan, four plan/work reviews, three `meta/inventories/` audits (removal set, suite audit, divergences), and the validation report.

## Testing

- [x] `mise run` (bare default — full local CI mirror incl. the entire test suite) exits 0 per the validation report; migrate integration suite alone reports 536 passed / 0 failed.
- [x] `mise run check` (read-only CI mirror across frontend, server, cli, build-system, scripts) exits 0.
- [x] `check-call-site-migration.sh`, `check-skill-permissions.sh`, and `check-inventory.sh` all exit 0 (removal-set floor 20, 22 divergence tests resolve, Phase-7 allowlist empty).
- [ ] Release-pipeline and live-session criteria (SessionStart hook envelope, binary download/verify) cannot run in this environment — deferred to a real session / release.

## Notes for Reviewers

- **Recorded, plan-approved deviations** to be aware of: `hooks.json` not inlined (thin `config-detect.sh` wrapper survives, deferred to 0169); `split-debuginfo` dropped; `ACCELERATOR_CACHE_DIR` left ungated; `store::current_umask()` hoisted; `libc` replaced by `rustix`.
- The validation result is **partial** only because a defined subset of the plan's own verification criteria remain unwritten and the release/live-session criteria can't execute here — every executable check is green.
- Highest blast radius in the epic: the SKILL.md/`allowed-tools` cutover touches 46 skill files uniformly. The `config_command` hexagon (`core/` vs `render/` split) and the `store` crate boundary are the highest-value areas to focus review on.
